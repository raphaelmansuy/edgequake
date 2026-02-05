# OODA Iteration 16 - Orient

## Analysis of Line Break Problem

### Pattern 1: Mid-word soft breaks

Lines ending with incomplete words followed by their continuation:

- Line ends with `netw`
- Next line starts with `orking`
- Detection: Previous line ends with lowercase letter (not punctuation), next line starts with lowercase

### Pattern 2: Hyphen-based word breaks

Explicit hyphenation markers for wrapped words:

- Line ends with word fragment + `-`
- Next line continues the word
- Detection: Line ends with `[a-z]-$`, next line starts with lowercase

### Pattern 3: Continuation indicators

Lines that clearly continue from previous:

- Line starts with lowercase letter
- Previous line doesn't end with sentence-ending punctuation (. ! ? :)
- Whitespace-indented continuations

## Strategy

Create `join_broken_lines()` function that:

1. **Detects soft word breaks**:
   - Previous line ends with `[a-z]` (no punctuation)
   - Current line starts with lowercase
   - Join with no space (word was split)

2. **Detects hyphenated breaks**:
   - Previous line ends with `[a-z]-\s*$`
   - Current line starts with lowercase
   - Join by removing the hyphen

3. **Preserves intentional breaks**:
   - Empty lines (paragraph boundaries)
   - Lines starting with markdown syntax (`#`, `-`, `*`, `|`, `>`)
   - Lines after punctuation (`. ! ? : ;`)
   - Code blocks

## Risks

- May incorrectly join unrelated lines
- Must preserve list structure
- Must not affect code blocks
- Hyphen removal might break legitimate compound words

## Mitigation

- Only join when previous line ends mid-word (lowercase, no punctuation)
- Preserve all markdown structural elements
- Test thoroughly with varied PDFs
