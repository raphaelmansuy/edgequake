# OODA Iteration 18 - Decide

## Decision: Fix Over-Aggressive Cross-Paragraph Joining in `join_broken_lines()`

### Problem Statement

The `join_broken_lines_single_pass()` function has a cross-empty-line join feature
(added in OODA-IT16) that joins text across `\n\n` paragraph breaks when the
pattern matches "lowercase ending → lowercase start". This feature was designed for
rare cases where PDF text boxes split a word like "netw\n\norking" across blocks
(with `render_text()` adding `\n\n` between them).

However, this feature causes **false positive joins** that damage document structure:

- `"...interleave the lines with\n\nsome space and tests..."` gets joined because
  "with" ends lowercase and "some" starts lowercase
- This destroys paragraph boundaries and produces long concatenated text blobs

### Root Cause

The `should_join_lines()` function uses a simple "lowercase→lowercase" heuristic
that cannot distinguish between:

1. A broken word fragment: `"netw"` (short, incomplete)
2. A complete sentence ending: `"...interleave the lines with"` (long, complete)

### Solution: Length Guard for Cross-Empty-Line Joins

**Add a line length threshold (≤30 chars) for cross-empty-line joins.**

First principles reasoning:

- PDF text boxes that split words are typically NARROW (short fragments)
- Complete sentences/paragraphs span the full column width (long lines)
- A 30-char threshold catches word fragments while rejecting full sentences

```
┌─────────────────────────────────────────────────────┐
│        CROSS-EMPTY-LINE JOIN DECISION TREE           │
├─────────────────────────────────────────────────────┤
│                                                      │
│  Current line ends lowercase?                        │
│  ├── NO  → Don't join                                │
│  └── YES                                             │
│       Next line is empty?                            │
│       ├── NO  → Apply normal should_join_lines()     │
│       └── YES                                        │
│            Current line ≤ 30 chars?  ← NEW GUARD     │
│            ├── NO  → Don't join (long = complete)    │
│            └── YES                                   │
│                 should_join_lines() matches?          │
│                 ├── NO  → Don't join                  │
│                 └── YES → Join (short fragment)       │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### Examples

| Current Line                     | Length | Next-Next Line           | Join?  |
| -------------------------------- | ------ | ------------------------ | ------ |
| `"netw"`                         | 4      | `"orking is prohibited"` | ✅ YES |
| `"TCP/IP netw"`                  | 11     | `"orking is prohibited"` | ✅ YES |
| `"...interleave the lines with"` | 50+    | `"some space and tests"` | ❌ NO  |
| `"sockets"`                      | 7      | `"based networking"`     | ✅ YES |

### Changes Required

1. **`src/renderers/markdown.rs`** - `join_broken_lines_single_pass()`:
   - Add `current_trimmed.len() <= 30` guard to cross-empty-line join condition
   - Add WHY comment explaining the threshold

2. **New tests:**
   - `test_join_broken_lines_no_cross_paragraph_join` - Long lines don't join across `\n\n`
   - `test_join_broken_lines_short_fragment_cross_empty` - Short fragments DO still join

### Risk Assessment

- **Low risk**: Only affects cross-empty-line joins, not same-line joins
- **No existing test breakage**: No test exercises cross-empty-line joining of long lines
- **Net positive**: Prevents false paragraph merging in ALL documents
