# Iteration 33 - Orient

**Date:** 2026-01-08  
**Focus:** Ambiguous glob re-exports cleanup

## Analysis

### Conflicting Functions Found

| Function         | Files                                                                  | Used by        |
| ---------------- | ---------------------------------------------------------------------- | -------------- |
| `default_stream` | chat_types.rs:17, conversations_types.rs:35, ollama_types.rs:107       | serde defaults |
| `default_limit`  | conversations_types.rs:15, graph_types.rs:114, workspaces_types.rs:160 | serde defaults |
| `default_true`   | documents_types.rs:41, entities_types.rs:19                            | serde defaults |

### Why This Matters

1. **Rust semantics:** When using `pub use module::*`, all public items are re-exported
2. **Name collision:** Same function names across modules cause ambiguity
3. **Serde requirement:** These are `#[serde(default = "...")]` helpers

### Solution Analysis

| Option                    | Pros            | Cons                   |
| ------------------------- | --------------- | ---------------------- |
| Rename to module-specific | Clear ownership | Many file changes      |
| Make `pub(crate)`         | Minimal changes | Still exported via `*` |
| Explicit re-exports       | Most control    | Verbose mod.rs         |
| **Keep pub, rename**      | **Best DX**     | Moderate changes       |

### Decision Criteria

- Minimize breaking changes
- Follow Rust conventions
- Improve code clarity
- Pass all tests

## Recommended Approach

Rename helper functions to be module-specific:

```rust
// Before:
pub fn default_stream() -> bool { true }

// After:
pub fn chat_default_stream() -> bool { true }
```

This makes ownership clear and eliminates ambiguity.
