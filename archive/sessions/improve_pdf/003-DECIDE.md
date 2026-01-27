# OODA Loop 3 - DECIDE

## Selected Fix: Filter Decorative Text Elements

### Change

**File:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**Function:** `extract_text_in_rect` (line ~717)

**Add filtering before concatenation:**

```rust
let mut text = String::new();
for (i, elem) in contained.iter().enumerate() {
    // Filter out decorative text (lines, borders, box-drawing chars)
    // First principles: Table cells contain alphanumeric content, not pure symbol runs
    let is_decorative = elem.text.len() > 1
        && elem.text.chars().all(|c| !c.is_alphanumeric() && !c.is_whitespace());

    if is_decorative {
        continue;  // Skip decorative elements
    }

    if i > 0 && !text.is_empty() {  // Only add space if text exists
        text.push(' ');
    }
    text.push_str(&elem.text);
}
```

### First Principles Justification

**Truth:** Table cells encode semantic information (words, numbers).  
**Observation:** PDF rendering sometimes uses text for graphics (lines, borders).  
**Solution:** Exclude text that has no alphanumeric characters.

**Safety:**

- ✅ Preserves "A1.2" (has alphanum)
- ✅ Preserves "0.4578" (has alphanum)
- ✅ Preserves "Scholarly" (has alphanum)
- ❌ Filters "---" (no alphanum, length > 1)
- ❌ Filters "│" (no alphanum if repeated)
- ✅ Preserves "." or "," (length = 1, below threshold)

### Predicted Impact

**Table Accuracy:** 2.4% → 60-70% (major improvement)

**Why:** This fixes the primary cell content corruption. The validator's token-level F1 will now match because extra `-` characters are removed.

### Edge Cases

**Potential false negatives:**

- Pure symbol cells (e.g., "★★★") → Filtered (acceptable tradeoff)
- Emoji-only cells → Filtered (rare in scientific PDFs)

**Mitigation:** Could lower threshold to length > 2 if needed.
