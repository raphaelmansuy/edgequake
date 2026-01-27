# OODA Loop 3 - OBSERVE

## Focus: Cell Content Pollution

### Problem

**Table Accuracy:** Still 2.4%

### Example Comparison

**Generated cell content:**

```
| Term Extraction | A1.2 - Scholarly | 0.4578 | 4 |
```

**Gold cell content:**

```
| Term Extraction | A1.2 Scholarly | 0.4578 | 4 |
```

**Difference:** Extra `-` characters in generated output.

### Root Cause

**File:** `lattice.rs` line ~719  
**Code:**

```rust
for (i, elem) in contained.iter().enumerate() {
    if i > 0 {
        text.push(' ');
    }
    text.push_str(&elem.text);  // <-- Problem: includes ALL text
}
```

**Issue:** TextElements can contain graphical characters:

- Horizontal lines rendered as `---` or `━━━`
- Vertical lines rendered as `|` or `│`
- Box-drawing characters: `┌`, `─`, `└`, etc.

### First Principles Analysis

**PDF text rendering:** Two types of content:

1. **Semantic text:** Words, numbers, meaningful content
2. **Decorative text:** Lines, borders, graphical elements rendered as text

**Current behavior:** Treats all TextElements equally  
**Desired behavior:** Filter decorative text, keep semantic content

### How to Distinguish?

**Pattern recognition:**

- Decorative text: `---`, `━━━`, `│`, `┌─┐`, etc.
- Long runs of repeated special characters
- Single-character elements that are box-drawing Unicode
- Text with very low alphanumeric ratio

**First principles rule:**  
Table cell content should be primarily alphanumeric. Lines of repeated special characters (---|━━━) are decorative, not content.
