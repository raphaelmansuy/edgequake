# IT12 Orient: Bullet List Detection Gap Analysis

## Root Cause Identified

The `BlockMergeProcessor` prevents merging blocks that start with bullets, but only when followed by a space:

```rust
// layout_processing.rs:271-274
if trimmed_b.starts_with("- ")
    || trimmed_b.starts_with("* ")
    || trimmed_b.starts_with("• ")  // ← Only catches "• text", NOT "•**text**"
```

**Problem:** In the LightRAG PDF, bullets are followed directly by bold markers:

- Extracted: `•**General Aspect**`
- Pattern: `• + ** + text + **`
- NO SPACE between bullet and bold markers

**Result:** `"•**General Aspect**"` does NOT match `"• "` pattern, so:

1. BlockMergeProcessor merges it with previous block
2. ListDetectionProcessor never sees it as a separate block
3. Bullet items become embedded in prose paragraphs

## Current Code Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     EXTRACTION                              │
├─────────────────────────────────────────────────────────────┤
│  PDF → Blocks: ["...In summary:", "•**General Aspect**..."] │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                   BlockMergeProcessor                       │
├─────────────────────────────────────────────────────────────┤
│  Check: trimmed_b.starts_with("• ")?                        │
│  "•**General Aspect**" does NOT start with "• "             │
│  → MERGE blocks together!                                   │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  ListDetectionProcessor                     │
├─────────────────────────────────────────────────────────────┤
│  Never sees bullet item as separate block                   │
│  → No list detection!                                       │
└─────────────────────────────────────────────────────────────┘
```

## Gap in `starts_with_bullet`

The `starts_with_bullet` function in `structure_detection.rs` DOES handle bullet without space:

```rust
fn starts_with_bullet(text: &str) -> bool {
    match chars.next() {
        None => true,  // Single bullet char is valid
        Some(' ') | Some('\t') => true,  // Bullet + space is valid
        _ => false,  // Bullet + other char is NOT valid
    }
}
```

But wait - this returns FALSE for `•**text` because the next char after `•` is `*`, not space.

## The Real Problem

Actually the `starts_with_bullet` logic is CORRECT:

- `• text` → bullet + space = list item ✅
- `•text` → bullet + letter = likely mathematical operator, not list ✅

But in the PDF, the formatting is:

- `•` + `**` (markdown bold) + `text`

The `**` should be RENDERING markers, not part of the text! This suggests the spans have bold=true and the \*\* is being added during rendering.

Let me check if the raw PDF text has the asterisks or if they're added by our renderer...

## Investigation Needed

1. What does the RAW extraction produce for bullet lines?
2. Are asterisks in the PDF or added by our bold rendering?
3. Should we strip formatting before bullet detection?

## Priority Fix

Update `BlockMergeProcessor.should_merge()` to also check:

- `"•**"` - bullet followed by bold markers
- More generally: bullet followed by any character (use `starts_with_bullet` helper)
