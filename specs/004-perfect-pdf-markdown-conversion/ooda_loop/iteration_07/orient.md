````markdown
# OODA-07 Orient: Root Cause Analysis

## Problem Statement

The regex `([a-z])([A-Z][a-z])` is too aggressive, splitting legitimate CamelCase terms.

## Analysis of Fix Attempts

### Option 1: Remove the regex entirely

**Pros:**

- No more CamelCase splitting
- Simple change

**Cons:**

- Loses the ability to fix genuine concatenation errors like `methodsThe`
- These concatenations DO occur in PDF extraction

### Option 2: Add whitelist of known CamelCase terms

**Pros:**

- Preserves both capabilities
- Explicit control

**Cons:**

- Maintenance burden
- Won't work for new/unknown terms
- Doesn't scale

### Option 3: Only split if preceding "word" is a common English word

**Pros:**

- More intelligent splitting
- `methodsThe` → `methods The` (methods is a word)
- `BrowseComp` → stays as is (Browse is not naturally followed by Comp)

**Cons:**

- Requires a dictionary
- Complex implementation
- False positives still possible

### Option 4: Use length-based heuristic

Only split if the lowercase portion before the uppercase is long (likely a complete word):

- `methodsThe` → split (methods = 7 chars, likely a word)
- `browseComp` → no split (browse = 6 chars, but starts with lowercase = likely CamelCase)

Actually, the key insight is:

- If the STRING STARTS with lowercase, it's probably sentence continuation → split
- If the STRING STARTS with uppercase (or is at word boundary), it's CamelCase → preserve

### Option 5: Only split at word boundaries

Current regex applies globally. Better approach:

- Only split if the pattern appears after a space/punctuation
- `text methodsThe model` → `text methods The model`
- `BrowseComp` → preserved (no space before B)

**Regex:** `(\s[a-z]+)([A-Z][a-z])` → `$1 $2`

This looks for space + lowercase letters + UpperLower and only splits there.

### Option 6: Add known CamelCase repair step AFTER splitting

Current code has:

```rust
result = result.replace("ar Xiv", "arXiv");
result = result.replace("Ar Xiv", "ArXiv");
```
````

We could add more known terms:

```rust
result = result.replace("Browse Comp", "BrowseComp");
result = result.replace("Deep Hallu Bench", "DeepHalluBench");
```

**Cons:** Doesn't scale to all papers.

## Decision: Option 5 (Word Boundary Approach)

The best solution is to make the splitting regex more conservative:

**Current:**

```rust
Regex::new(r"([a-z])([A-Z][a-z])")
```

**Proposed:**

```rust
Regex::new(r"(\s)([a-z]+)([A-Z][a-z])")
// Only matches: space + lowercase word + UpperLower
// Replacement: "$1$2 $3"
```

This will:

- ✅ Fix: `text methodsThe model` → `text methods The model`
- ✅ Preserve: `BrowseComp` (no leading space before B)
- ✅ Preserve: `DeepHalluBench` (no leading space before D)
- ✅ Preserve: `ArXiv` (no lowercase before A)

## Files to Modify

1. `text_cleanup.rs:377-380` - Update regex pattern
2. Add test cases for CamelCase preservation

```

```
