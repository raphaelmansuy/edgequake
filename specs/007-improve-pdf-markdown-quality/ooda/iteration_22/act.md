# OODA Iteration 22 – Act

## Changes made

### 1. Inter-span word gap detection (`pdfium_backend.rs`)

Added gap detection between consecutive spans within the same line:

```rust
if span_idx > 0 {
    let prev = &line.spans[span_idx - 1];
    let gap = span.x0 - prev.x1;
    let avg_size = (prev.font_size + span.font_size) / 2.0;
    let space_threshold = avg_size * 0.15;  // Same as Line::text()
    let starts_with_hyphen = span.text.starts_with('-') || ...;
    let ends_with_hyphen = prev.text.ends_with('-') || ...;
    if gap > space_threshold && !starts_with_hyphen && !ends_with_hyphen {
        block.spans.push(TextSpan::plain(" "));
    }
}
```

### 2. Inter-line separator change (`pdfium_backend.rs`)

Changed line 575 from:

```rust
block.spans.push(TextSpan::plain("\n"));
```

To:

```rust
block.spans.push(TextSpan::plain(" "));
```

With WHY comment explaining the trim() interaction in `render_spans_styled()`.

### 3. Test update

Updated `test_convert_block_preserves_spans` to expect 3 spans (Bold + space + Normal) instead of 2.

## Verification

- 569 tests pass (0 failures)
- Zero new clippy warnings
- Visual verification on AI_Services_Elitizon.pdf:
  - ✅ "AI Services" (was "AIServices")
  - ✅ "Executive summary" (was "Executivesummary")
  - ✅ "build vs-buy" (was "buildvs-buy")
  - ✅ "from pilot" (was "frompilot")
  - ✅ "that produce" (was "thatproduce")
  - ✅ "agentic workflows" (was "agenticworkflows")
  - ✅ "help teams" (was "helpteams")
  - ✅ "evaluation protocol" (was "evaluationprotocol")
  - ✅ "customer support" (was "customersupport")
  - ✅ "existing code" (was "existingcode")

## Commit

`Fix word spacing: inter-span gaps + line separator \n→space (OODA-IT22)`
