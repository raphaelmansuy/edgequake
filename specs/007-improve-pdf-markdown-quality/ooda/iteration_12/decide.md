# IT12 Decide: Improve Bullet Detection Without Space

## Decision

Update `starts_with_bullet()` to accept bullets followed by uppercase letters.

## Rationale

```
┌─────────────────────────────────────────────────────────────┐
│                    DECISION ANALYSIS                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CURRENT: "•X" → NOT bullet (requires space)                │
│  WANTED:  "•X" → IS bullet if X is uppercase                │
│                                                             │
│  WHY uppercase: List items start sentences (capital letter) │
│  WHY NOT any letter: "∙ome" could be mathematical symbol    │
│                                                             │
│  SAFE PATTERNS:                                             │
│  - "• text"     → bullet + space → KEEP (existing)          │
│  - "•General"   → bullet + uppercase → ADD (new)            │
│  - "•**bold**"  → bullet + asterisk → ADD (markdown bold)   │
│                                                             │
│  UNSAFE PATTERNS (reject):                                  │
│  - "•x"         → bullet + lowercase → math operator        │
│  - "•123"       → bullet + digit → not a list item          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Step 1: Update `starts_with_bullet()` in structure_detection.rs

```rust
match chars.next() {
    None => true,
    Some(' ') | Some('\t') => true,
    Some('*') => true,  // NEW: Allow markdown bold markers
    Some(c) if c.is_uppercase() => true,  // NEW: Allow uppercase (sentence start)
    _ => false,
}
```

### Step 2: Update `BlockMergeProcessor.should_merge()` in layout_processing.rs

Use `starts_with_bullet()` helper instead of manual `starts_with("• ")` check:

```rust
// Import or inline the bullet check logic
let is_bullet = starts_with_bullet(trimmed_b);
if is_bullet || trimmed_b.starts_with("- ") || ... {
    return false;  // Don't merge
}
```

### Step 3: Add tests for new patterns

```rust
#[test]
fn test_starts_with_bullet_uppercase() {
    assert!(starts_with_bullet("•General Aspect"));
    assert!(starts_with_bullet("•Agriculture: This domain"));
}

#[test]
fn test_starts_with_bullet_markdown_bold() {
    assert!(starts_with_bullet("•**Bold text**"));
}
```

## Expected Outcome

- `•General Aspect` detected as list item ✅
- `•Agriculture:` detected as list item ✅
- `•**Bold**` detected as list item ✅
- `∙ome` NOT detected (lowercase) ✅
- Math symbols NOT detected ✅
