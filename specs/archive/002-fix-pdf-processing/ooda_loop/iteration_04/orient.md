# OODA-04 Orient Phase

## Date: 2026-01-31
## Context Analysis

## Problem Classification

### Issue Type 1: UTF-8 String Boundary Violation (3 files)
- **Severity**: Critical (causes panics)
- **Pattern**: Debug logging uses `&text[..n]` or `&text[..min(len, n)]`
- **Root Cause**: Rust strings are UTF-8 encoded; slicing at arbitrary byte positions can split multibyte characters

### Issue Type 2: Tokio Runtime Context Missing (1 file)
- **Severity**: Critical (causes panics)
- **Pattern**: `tokio::spawn` called from Rayon worker thread
- **Root Cause**: Rayon thread pool is synchronous, lacks Tokio runtime context

## Technical Context

### UTF-8 Multibyte Characters
PDF documents frequently contain:
- Unicode characters (Ç, é, ñ, ü, etc.)
- Mathematical symbols (∑, ∫, ∂, etc.)
- CJK characters (中文, 日本語, 한국어)
- Special punctuation (—, –, ', ", etc.)

These encode as 2-4 bytes in UTF-8. Slicing at byte 80 when a character spans bytes 79-81 causes panic.

### Rayon vs Tokio Architecture
```
┌─────────────────────────────────────────────────────────┐
│ Axum Handler (Tokio runtime context available)          │
│   ↓                                                     │
│ spawn_blocking → enters Rayon thread pool               │
│   ↓                                                     │
│ PDF Extraction (sync code, no Tokio runtime)            │
│   ↓                                                     │
│ PipelineProgressCallback.on_progress()                  │
│   ↓                                                     │
│ tokio::spawn() ← PANIC! No reactor running              │
└─────────────────────────────────────────────────────────┘
```

## Solution Approaches

### UTF-8 Fix Strategy
Reuse the `safe_truncate()` helper pattern from OODA-03:

```rust
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
```

### Tokio Runtime Fix Strategy
Capture `Handle::current()` at construction time when in Tokio context:

```rust
struct PipelineProgressCallback {
    runtime_handle: tokio::runtime::Handle,
    // ... other fields
}

impl PipelineProgressCallback {
    pub fn new(...) -> Self {
        Self {
            runtime_handle: Handle::current(), // Capture here
            // ...
        }
    }
}
```

Then use `self.runtime_handle.spawn()` instead of `tokio::spawn()`.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Missing additional slicing locations | Low | High | grep for `&.*\[\.\.` patterns |
| Handle capture fails | Very Low | High | Handle::current() panics if no runtime |
| Performance impact | Very Low | Low | No measurable overhead |

## Decision Criteria
1. Zero panics for any UTF-8 input
2. Progress updates work from sync context
3. Minimal code changes (DRY principle)
4. Each file gets its own helper to avoid module dependencies
