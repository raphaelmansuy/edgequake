# OODA-20 Decide: Prioritized Actions for UTF-8 Safety

## Date: 2025-02-03

## Decision Matrix

| Priority | Action | Impact | Effort | Status |
|----------|--------|--------|--------|--------|
| P0 | Fix extraction_engine.rs:656 panic | Critical | Low | ✅ DONE |
| P0 | Fix layout_processing.rs unsafe slices | Critical | Low | ✅ DONE |
| P1 | Verify fix with problematic PDF | High | Low | 🔄 IN PROGRESS |
| P1 | Run comprehensive tests | High | Medium | 🔄 PENDING |
| P2 | Audit codebase for similar issues | Medium | Medium | 📋 PLANNED |

## Decisions Made

### Decision 1: Use chars().take(n) for New Code ✅

**Rationale:**
- Idiomatic Rust pattern
- Zero dependencies
- Clear intent

**Applied to:**
- extraction_engine.rs:656

```rust
// Before (UNSAFE):
if blk.text.len() > 45 { &blk.text[..45] } else { &blk.text }

// After (SAFE):
let truncated: String = blk.text.chars().take(45).collect();
```

### Decision 2: Use Existing safe_truncate() Where Available ✅

**Rationale:**
- Function already exists in layout_processing.rs
- Avoids code duplication
- Consistent patterns within file

**Applied to:**
- layout_processing.rs:111, 629, 705

### Decision 3: Document Why in Comments ✅

**Rationale:**
- Prevent future regressions
- Educational for contributors
- Clear intent for code reviewers

**Example:**
```rust
// WHY: Use char_indices to safely truncate UTF-8 strings at character boundaries
// because direct byte slicing (e.g., &text[..45]) can panic on multi-byte characters
// like curly quotes (' ' " ") which are 3 bytes each in UTF-8
```

## Verification Plan

1. **Build verification** ✅
   - `cargo build --package edgequake-pdf` passes

2. **Smoke test** ✅
   - `cargo test --package edgequake-pdf --test quick_smoke` passes (4/4)

3. **Feature test** (pending)
   - `cargo test --package edgequake-pdf --test basic_features --features slow-tests`

4. **Target PDF test** (pending)
   - Extract `agentfail_2601.22984v1.pdf` without crash

5. **Comprehensive test** (pending)
   - Full quality suite when time permits

## Commit Strategy

**Commit message:**
```
OODA-20: Fix UTF-8 panic on multi-byte character slicing

Problem: Debug output used direct byte slicing (&text[..45]) which
panics when the index falls inside a multi-byte UTF-8 character.
This broke extraction of academic papers with curly quotes.

Solution: Use safe truncation methods:
- chars().take(n).collect() for new code
- safe_truncate() helper for existing patterns

Files changed:
- extraction_engine.rs: Fix debug output at line 656
- layout_processing.rs: Fix debug output at lines 111, 629, 705

Impact: Fixes crashes on documents with smart typography
(curly quotes, em-dashes, international text).
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Performance regression | chars().take(n) is O(n) but n<100, negligible |
| Incomplete fix | Run comprehensive tests before committing |
| Future regressions | WHY comments explain the pattern |
