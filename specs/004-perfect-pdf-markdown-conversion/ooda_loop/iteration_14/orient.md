# OODA-14: Orient

## Root Cause Analysis

### Problem 1: Numbered Lists Without Spacing

Some PDFs encode numbered lists as "1.Text" without space after the period.
Our regex `r"^\d+[\.)]\s+"` requires the space, missing these patterns.

### Problem 2: Section Header Collision

Making space optional with `\s*` causes "1.1 Title" section headers to match.
These should NOT be converted to list items.

### Problem 3: Regex Lookahead Not Supported

Attempted fix with `(?!\d)` negative lookahead to exclude decimal patterns.
Rust's regex crate doesn't support lookahead, causing runtime panic.

## First Principles Approach

**Numbered List Pattern Analysis:**

- Valid list: `1. Text` or `1.Text` where next char is NOT a digit
- Invalid (section): `1.1 Title` where period is followed by another digit

**Constraint:** Cannot use lookahead in Rust regex

**Alternative Approach:**

1. Keep original regex requiring space for standard lists
2. Add SECONDARY regex for "digit.UPPERCASE" pattern
3. This catches "1.Explore" but NOT "1.1 Title"

## Strategy

Use TWO regex patterns:

1. `r"^\d+[\.)]\s+"` - Standard list with space (original)
2. `r"^\d+\.[A-Z]"` - No-space list with uppercase start (new)

The second pattern specifically requires an uppercase letter after the period,
which matches list starts ("1.Explore") but NOT section numbers ("1.1").

## Impact Assessment

- **Low Risk**: Changes are additive, original pattern preserved
- **Medium Complexity**: Two patterns to maintain
- **High Value**: Catches edge case without breaking existing detection
