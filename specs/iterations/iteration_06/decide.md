# OODA-06 Decide: Preserve Line Breaks in Paragraphs

## Decision

Change paragraph line rendering from space-joined to newline-joined.

## Change

**File**: `layout/pymupdf_renderer.rs`
**Function**: `render_lines_inline`
**Line**: ~156

**Before**:
```rust
.join(" ")
```

**After**:
```rust
.join("\n")
```

## Rationale

1. pymupdf4llm preserves line breaks within paragraphs
2. Gold files show ~80 char lines, not long single lines
3. Lines ratio is 0.525 (620 vs 1181) - worst component of Structure score
4. Simple fix with minimal risk

## Verification

1. Build: `cargo build --release --features pdfium -p edgequake-pdf`
2. Test: `python3 scripts/eval_comprehensive.py`
3. Target: Structure > 0.60, Quality > 0.73
