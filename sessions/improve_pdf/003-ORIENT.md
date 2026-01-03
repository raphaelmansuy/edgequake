# OODA Loop 3 - ORIENT

## Root Cause: Decorative Text in Cells

### First Principles Analysis

**Mathematical definition of table cell content:**

- Primary: Alphanumeric tokens (words, numbers)
- Secondary: Punctuation attached to alphanumerics (e.g., "0.95", "A1.2")
- **NOT:** Standalone special character runs

**Decorative pattern signatures:**

1. **Horizontal lines:** `---`, `━━━`, `═══`, `___`
2. **Vertical lines:** `|||`, `│││`
3. **Box drawing:** `┌`, `├`, `└`, `─`, `│`, etc.
4. **Mixed:** `|--|`, `+--+`

**Detection heuristic (first principles):**

- If text consists entirely of non-alphanumeric characters
- AND length > 2 (not punctuation like ".", ",")
- THEN it's decorative

### Alternative Approaches

#### Option 1: Character-type ratio filter

```rust
let alnum_count = text.chars().filter(|c| c.is_alphanumeric()).count();
let ratio = alnum_count as f32 / text.len() as f32;
if ratio < 0.3 { skip }  // <30% alphanumeric = decorative
```

**Pros:** Catches all decorative patterns  
**Cons:** Might filter legitimate punctuation-heavy content

#### Option 2: Exact pattern matching

```rust
if text.chars().all(|c| matches!(c, '-' | '_' | '|' | '=' | '+')) {
    skip
}
```

**Pros:** Precise, fast  
**Cons:** Misses Unicode box-drawing chars

#### Option 3: Unicode category check

```rust
if text.chars().all(|c| !c.is_alphanumeric() && !c.is_whitespace()) {
    skip
}
```

**Pros:** Language-agnostic, handles Unicode  
**Cons:** Might filter pure-symbol content (e.g., "★★★")

### Selected Approach: Option 3 (Hybrid)

**Logic:**

1. If text is entirely non-alphanumeric (excluding whitespace)
2. AND text length > 1 (preserve single punctuation)
3. THEN skip it

**Rationale:** Most robust, handles both ASCII and Unicode decorations.
